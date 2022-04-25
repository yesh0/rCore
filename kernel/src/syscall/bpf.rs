use super::{SysError::*, *};

use crate::bpf::consts::*;
use crate::bpf::map::*;

impl Syscall<'_> {
    pub fn sys_bpf(&self, cmd: usize, attr_ptr: usize, _size: usize) -> SysResult {
        // error!("sys_bpf cmd = {}", cmd);
        match cmd {
            BPF_MAP_CREATE => {
                let ptr = UserInPtr::<MapAttr>::from(attr_ptr);
                let map_attr = ptr.read()?;
                // error!("map create: {:?}", map_attr);
                bpf_map_create(map_attr)
            }
            BPF_MAP_LOOKUP_ELEM | BPF_MAP_UPDATE_ELEM | BPF_MAP_DELETE_ELEM
            | BPF_MAP_GET_NEXT_KEY => {
                let ptr = UserInPtr::<MapOpAttr>::from(attr_ptr);
                let op_attr = ptr.read()?;
                let map_attr = bpf_map_get_attr(op_attr.map_fd).ok_or(ENOENT)?;
                self.handle_map_ops(cmd, op_attr, map_attr)
            }
            _ => Err(EINVAL),
        }
    }

    fn handle_map_ops(&self, op: usize, op_attr: MapOpAttr, map_attr: InternalMapAttr) -> SysResult {
        let vm = self.vm();
        // pointers
        let key = op_attr.key as *const u8;
        let value = op_attr.value as *mut u8;
        // sizes
        let key_sz = map_attr.key_size;
        let val_sz = map_attr.value_size;
        // we always need to read the key
        let _ = unsafe { vm.check_read_array(key, key_sz)? };
        match op {
            BPF_MAP_LOOKUP_ELEM => {
                let _ = unsafe { vm.check_write_array(value, val_sz)? };
            }
            BPF_MAP_UPDATE_ELEM => {
                let _ = unsafe { vm.check_read_array(value, val_sz)? };
            }
            BPF_MAP_GET_NEXT_KEY => {
                let _ = unsafe { vm.check_write_array(value, key_sz)? };
            }
            _ => {}
        }
        bpf_map_ops(op_attr.map_fd, op, key, value, op_attr.flags)
    }
}
