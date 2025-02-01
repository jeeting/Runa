// shoutout to chatgpt
use crate::memory::Luna;
use crate::propertydescriptor::PropertyDescriptorContainer;

pub struct ClassDescriptor {
    address: usize,
}

impl ClassDescriptor {
    pub fn new(address: usize) -> Self {
        ClassDescriptor { address }
    }

    pub fn name(&self, mem: &Luna) -> String {
        let name_pointer = mem.read_pointer(self.address + 0x8);
        mem.read_rbx_str(name_pointer).unwrap_or_else(|| String::from("Unknown"))
    }

    pub fn property_descriptors(&self, mem: &Luna) -> PropertyDescriptorContainer {
        PropertyDescriptorContainer::new(self.address)
    }
}
