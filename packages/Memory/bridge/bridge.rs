// shoutout to chatgpt
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use regex::Regex;
use serde::{Serialize, Deserialize};
use serde_json::json;

const DATA_MAX_LEN: usize = 199998;
const PAYLOAD_MATCH: &str = "^[A-Fa-f0-9]{8}";

#[derive(Clone, Copy)]
pub enum PeerType {
    Roblox = 0,
    External = 1,
}

#[derive(Clone, Copy)]
pub enum SenderType {
    R2E = 0, // Roblox to external
    E2R = 1,  // External to Roblox
}

fn extract_bits(value: u32, field: usize, width: u32) -> u32 {
    (value >> field) & width
}

pub struct BridgeChannel {
    handle: i32,
    name: String,
    states: Instance,
    peer0: Instance,
    peer1: Instance,
    instance_refs: Instance,
    buffers_caches: HashMap<i32, HashMap<i32, Instance>>,
}

impl BridgeChannel {
    pub fn new(handle: i32, name: String, peer0: Instance, peer1: Instance) -> Self {
        BridgeChannel {
            handle,
            name,
            states: Instance::new(),
            peer0,
            peer1,
            instance_refs: Instance::new(),
            buffers_caches: HashMap::new(),
        }
    }

    pub fn initialize(&mut self, channel_container: Instance) {
        self.name = channel_container.name();
        self.states = channel_container.wait_for_child("States", 1);
        self.peer0 = channel_container.wait_for_child("Peer0", 1);
        self.peer1 = channel_container.wait_for_child("Peer1", 1);
        self.instance_refs = channel_container.wait_for_child("InstanceRefs", 1);
    }

    pub fn get_channel_states(&self) -> (bool, bool, bool, Option<i32>) {
        if self.states.address == 0 || self.states.value().is_none() {
            return (false, false, false, None);
        }

        let data = self.states.value().unwrap();
        if data.is_empty() {
            return (false, false, false, None);
        }

        let packed_value = self.states.value().unwrap().parse::<i32>().unwrap();

        let is_used = extract_bits(packed_value as u32, 0, 1) == 1;
        let responding = extract_bits(packed_value as u32, 1, 1) == 1;
        let responded = extract_bits(packed_value as u32, 2, 1) == 1;
        let sender = extract_bits(packed_value as u32, 3, 1);

        let sender_ptr = if is_used { Some(sender) } else { None };

        (is_used, responding, responded, sender_ptr)
    }

    pub fn set_channel_states(&mut self, is_used: bool, responding: bool, responded: bool, sender: i32) {
        if self.states.address < 1000 {
            return;
        }

        let mut result = 0;
        if is_used {
            result |= 0b0001;
        }
        if responding {
            result |= 0b0010;
        }
        if responded {
            result |= 0b0100;
        }
        result |= (sender << 3) & 0b1000;
        self.states.set_value(result);
    }

    pub fn get_buffer_data(&self, container_type: i32) -> String {
        let container = if container_type == 0 {
            &self.peer0
        } else if container_type == 1 {
            &self.peer1
        } else {
            return String::new();
        };

        let mut result = String::new();

        for buffer_idx in 0..container.children().len() {
            let buffer_obj = if let Some(val) = self.buffers_caches.get(&container_type).unwrap().get(&buffer_idx) {
                val
            } else {
                container.find_first_child(&buffer_idx.to_string(), true)
            };

            if buffer_obj.address > 1000 && buffer_obj.value().is_some() {
                result.push_str(&buffer_obj.value().unwrap());
            }
        }

        if result.is_empty() {
            return String::new();
        }

        let re = Regex::new(PAYLOAD_MATCH).unwrap();
        if let Some(buffer_size_match) = re.find(&result) {
            let match_len = buffer_size_match.start();
            let buffer_size: usize = usize::from_str_radix(&result[match_len..match_len + 8], 16).unwrap();
            let start_idx = match_len + 1;
            let end_idx = start_idx + buffer_size + (buffer_size / DATA_MAX_LEN) + 1;

            if end_idx > result.len() {
                return result[start_idx..].to_string();
            }
        }

        String::new()
    }

    pub fn set_buffer_data(&mut self, new_data: String) -> bool {
        let buffers_cache = self.buffers_caches.entry(PeerType::External as i32).or_insert_with(HashMap::new);

        for buffer_pos in (0..new_data.len()).step_by(DATA_MAX_LEN) {
            let buffer_idx = buffer_pos / DATA_MAX_LEN;

            let buffer_obj = if let Some(val) = buffers_cache.get(&buffer_idx) {
                val
            } else {
                self.peer1.find_first_child(&buffer_idx.to_string(), true)
            };

            if buffer_obj.address <= 1000 {
                return false;
            }

            let end_pos = std::cmp::min(buffer_pos + DATA_MAX_LEN, new_data.len());
            buffer_obj.set_value(new_data[buffer_pos..end_pos].to_string());
        }

        true
    }
}

pub struct Bridge {
    channels: Vec<BridgeChannel>,
    sessions: HashMap<String, i32>,
    queued_datas: Vec<String>,
    callbacks_registry: HashMap<String, Box<dyn Fn(i32, Vec<serde_json::Value>) -> Vec<serde_json::Value>>>,
    roblox_terminated: bool,
    main_container: Instance,
    module_holder: Instance,
    mutex: Arc<Mutex<()>>,
}

impl Bridge {
    pub fn new() -> Self {
        Bridge {
            channels: Vec::new(),
            sessions: HashMap::new(),
            queued_datas: Vec::new(),
            callbacks_registry: HashMap::new(),
            roblox_terminated: false,
            main_container: Instance::new(),
            module_holder: Instance::new(),
            mutex: Arc::new(Mutex::new(())),
        }
    }

    pub fn start(&mut self, new_pid: i32, main_container: Instance) {
        self.main_container = main_container;
        self.module_holder = self.main_container.wait_for_child("ModuleHolder", 5);
        let channels = self.main_container.wait_for_child("Channels", 5);

        if self.module_holder.address < 1000 || channels.address < 1000 {
            return;
        }

        self.channels = Vec::new();
        self.sessions = HashMap::new();
        self.queued_datas = Vec::new();
        self.callbacks_registry = HashMap::new();

        for channel_idx in 0..8 {
            let channel_container = channels.find_first_child(&channel_idx.to_string(), false);
            if channel_container.address < 1000 {
                continue;
            }
            let mut channel_obj = BridgeChannel::new(new_pid, "channel_name".to_string(), Instance::new(), Instance::new());
            channel_obj.initialize(channel_container);
            self.channels.push(channel_obj);

            thread::sleep(Duration::from_millis(50));
        }

        self.roblox_terminated = false;
    }

    pub fn send(&mut self, action: String, args: Vec<serde_json::Value>) {
        let last = Instant::now();

        if self.roblox_terminated {
            return;
        }

        let _lock = self.mutex.lock().unwrap();

        let session = self.sessions.entry(action.clone()).or_insert(0);
        let payload = self.process_data(&action, *session, &args).unwrap();

        self.queued_datas.push(payload);
        self.sessions.insert(action.clone(), *session + 1);

        if Instant::now().duration_since(last) <= Duration::from_millis(50) {
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn register_callback<F>(&mut self, callback_name: String, callback: F)
    where
        F: 'static + Fn(i32, Vec<serde_json::Value>) -> Vec<serde_json::Value>,
    {
        self.callbacks_registry.insert(callback_name, Box::new(callback));
    }

    fn process_data(&self, action: &str, session: i32, args: &[serde_json::Value]) -> Result<String, serde_json::Error> {
        let data = json!([action, session, args]);
        let result = serde_json::to_string(&data)?;
        let data_len_hex = format!("{:08x}", result.len());
        Ok(format!("{}|{}", data_len_hex, result))
    }

    pub fn bridge_listener(&mut self) {
        loop {
            thread::sleep(Duration::from_millis(1));
            if self.roblox_terminated {
                break;
            }

            for channel in &self.channels {
                let (is_used, _, _, sender) = channel.get_channel_states();

                if sender == Some(SenderType::E2R as i32) || !is_used {
                    continue;
                }

                let raw_data = channel.get_buffer_data(PeerType::Roblox as i32);

                if raw_data.is_empty() {
                    channel.set_channel_states(false, false, false, SenderType::E2R as i32);
                    continue;
                }

                let received_data: Vec<serde_json::Value> = serde_json::from_str(&raw_data).unwrap();

                if received_data.len() < 3 {
                    channel.set_channel_states(false, false, false, SenderType::E2R as i32);
                    continue;
                }

                let action = received_data[0].as_str().unwrap();
                let session = received_data[1].as_i64().unwrap() as i32;
                let raw_args = received_data[2].as_array().unwrap();

                if let Some(callback) = self.callbacks_registry.get(action) {
                    let mut action_args = Vec::new();
                    for value_info in raw_args {
                        let value_slice = value_info.as_array().unwrap();
                        if value_slice.len() < 2 {
                            continue;
                        }
                        let value_type = value_slice[0].as_str().unwrap();
                        let mut value = value_slice[1].clone();
                        if value_type == "Instance" {
                            value = channel.instance_refs.find_first_child(&value.to_string(), true).value().unwrap();
                        } else if value_type == "table" {
                            let value_str = value.to_string();
                            let table_value: serde_json::Value = serde_json::from_str(&value_str).unwrap();
                            value = table_value;
                        }
                        action_args.push(value);
                    }

                    self.handle_callback(action, channel, callback, session, action_args);
                    channel.set_channel_states(true, true, false, SenderType::E2R as i32);
                } else {
                    channel.set_channel_states(false, false, false, SenderType::E2R as i32);
                }
            }
        }
    }

    fn handle_callback(&self, cbname: &str, channel: &BridgeChannel, callback: &dyn Fn(i32, Vec<serde_json::Value>) -> Vec<serde_json::Value>, session: i32, args: Vec<serde_json::Value>) {
        let returned_args = callback(session, args);
        let payload = self.process_data(cbname, session, &returned_args).unwrap();
        let set_success = channel.set_buffer_data(payload);
        channel.set_channel_states(set_success, false, true, SenderType::E2R as i32);
    }

    pub fn bridge_queue_sched(&mut self) {
        loop {
            thread::sleep(Duration::from_millis(1));

            if self.roblox_terminated || self.queued_datas.is_empty() {
                continue;
            }

            if let Some(channel) = self.get_available_channel() {
                let _lock = self.mutex.lock().unwrap();
                let payload = self.queued_datas.remove(0);

                let set_success = channel.set_buffer_data(payload);
                channel.set_channel_states(set_success, false, false, SenderType::E2R as i32);
            }
        }
    }

    pub fn get_available_channel(&self) -> Option<&BridgeChannel> {
        for channel in &self.channels {
            let (is_used, _, _, _) = channel.get_channel_states();
            if !is_used {
                return Some(channel);
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct Instance {
    pub address: i32,
    // Other fields
}

impl Instance {
    pub fn new() -> Self {
        Instance { address: 0 }
    }

    pub fn name(&self) -> String {
        "InstanceName".to_string()
    }

    pub fn wait_for_child(&self, child_name: &str, retries: i32) -> Instance {
        // Example method, just return new instance
        Instance::new()
    }

    pub fn children(&self) -> Vec<Instance> {
        vec![]
    }

    pub fn find_first_child(&self, name: &str, create_if_needed: bool) -> Instance {
        Instance::new()
    }

    pub fn value(&self) -> Option<String> {
        Some("example_value".to_string())
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }
}

fn main() {
    let bridge = Bridge::new();
}
