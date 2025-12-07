use crate::{
    memory::{Handle, MemObject},
    types::{
        object::{string::SelfString, vector::Vector},
        Value,
    },
    vm::Vm,
};

pub fn put_string(vm: &mut Vm, string: String) -> Handle {
    vm.memory.alloc(MemObject::String(SelfString::new(string)))
}

pub fn put_vector(vm: &mut Vm, vector: Vec<Value>) -> Handle {
    vm.memory
        .alloc(MemObject::Vector(Vector::new_initialized(vector, vm)))
}
