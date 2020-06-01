/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use cpython::{PyBytes, PyModule, PyObject, PyResult, Python};
use revlogindex::NodeRevMap;
use revlogindex::RevlogEntry;
use std::slice;

use cpython_ext::ResultPyErrExt;
use cpython_ext::SimplePyBuf;

pub fn init_module(py: Python, package: &str) -> PyResult<PyModule> {
    let name = [package, "indexes"].join(".");
    let m = PyModule::new(py, &name)?;
    m.add_class::<nodemap>(py)?;
    Ok(m)
}

py_class!(class nodemap |py| {
    data nodemap: NodeRevMap<SimplePyBuf<RevlogEntry>, SimplePyBuf<u32>>;

    def __new__(_cls, changelog: &PyObject, index: &PyObject) -> PyResult<nodemap> {
        let changelog_buf = SimplePyBuf::new(py, changelog);
        let index_buf = SimplePyBuf::new(py, index);
        let nm = NodeRevMap::new(changelog_buf, index_buf).map_pyerr(py)?;
        nodemap::create_instance(py, nm)
    }

    def __getitem__(&self, key: PyBytes) -> PyResult<Option<u32>> {
        Ok(self.nodemap(py).node_to_rev(key.data(py)).map_pyerr(py)?)
    }

    def __contains__(&self, key: PyBytes) -> PyResult<bool> {
        Ok(self.nodemap(py).node_to_rev(key.data(py)).map_pyerr(py)?.is_some())
    }

    def partialmatch(&self, hex: &str) -> PyResult<Option<PyBytes>> {
        Ok(self.nodemap(py).hex_prefix_to_node(hex).map_pyerr(py)?.map(|b| PyBytes::new(py, b)))
    }

    def build(&self) -> PyResult<PyBytes> {
        let buf = self.nodemap(py).build_incrementally().map_pyerr(py)?;
        let slice = unsafe { slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4) };
        Ok(PyBytes::new(py, slice))
    }

    def lag(&self) -> PyResult<u32> {
        Ok(self.nodemap(py).lag())
    }

    @staticmethod
    def emptyindexbuffer() -> PyResult<PyBytes> {
        let buf = NodeRevMap::<Vec<RevlogEntry>, Vec<u32>>::empty_index_buffer();
        let slice = unsafe { slice::from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4) };
        Ok(PyBytes::new(py, slice))
    }
});
