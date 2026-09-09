use libmpv2_sys as sys;
use std::ffi::CStr;

struct Node(sys::mpv_node);
impl Drop for Node {
    fn drop(&mut self) {
        unsafe { sys::mpv_free_node_contents(&mut self.0) };
    }
}

// vf is a native array of maps, not a sub-property list with vf/count. Read one
// atomic snapshot and free mpv's allocation after all borrowed fields are used.
pub(super) fn filters(mpv: &libmpv2::Mpv) -> Result<Vec<(String, bool)>, String> {
    let mut node = Node(unsafe { std::mem::zeroed() });
    let result = unsafe {
        sys::mpv_get_property(
            mpv.ctx.as_ptr(),
            c"vf".as_ptr(),
            sys::mpv_format_MPV_FORMAT_NODE,
            (&mut node.0 as *mut sys::mpv_node).cast(),
        )
    };
    if result < 0 {
        return Err(format!("cannot read native filter status: {result}"));
    }
    // All pointers below belong to the live mpv-allocated Node; formats are
    // checked before union access. mpv guarantees list lengths and C strings.
    unsafe {
        if node.0.format != sys::mpv_format_MPV_FORMAT_NODE_ARRAY {
            return Err("invalid filter list".into());
        }
        let list = &*node.0.u.list;
        let mut filters = Vec::new();
        for i in 0..list.num {
            let entry = &*list.values.add(i as usize);
            if entry.format != sys::mpv_format_MPV_FORMAT_NODE_MAP {
                return Err("invalid filter entry".into());
            }
            let fields = &*entry.u.list;
            let mut label = None;
            let mut enabled = None;
            for j in 0..fields.num {
                let key = CStr::from_ptr(*fields.keys.add(j as usize)).to_bytes();
                let value = &*fields.values.add(j as usize);
                if key == b"label" && value.format == sys::mpv_format_MPV_FORMAT_STRING {
                    label = Some(
                        CStr::from_ptr(value.u.string)
                            .to_string_lossy()
                            .into_owned(),
                    );
                } else if key == b"enabled" && value.format == sys::mpv_format_MPV_FORMAT_FLAG {
                    enabled = Some(value.u.flag != 0);
                }
            }
            if let Some(label) = label {
                filters.push((label, enabled.unwrap_or(false)));
            }
        }
        Ok(filters)
    }
}
