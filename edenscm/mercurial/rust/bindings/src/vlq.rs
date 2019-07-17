// Copyright 2019 Facebook, Inc.
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

use cpython::*;
use cpython_failure::ResultPyErrExt;
use vlqencoding::{VLQDecode, VLQDecodeAt, VLQEncode};

pub fn init_module(py: Python, package: &str) -> PyResult<PyModule> {
    let name = [package, "vlq"].join(".");
    let m = PyModule::new(py, &name)?;
    m.add(
        py,
        "decodeat",
        py_fn!(py, decodeat(data: PyBytes, offset: usize)),
    )?;
    m.add(py, "decode", py_fn!(py, decode(data: PyBytes)))?;
    m.add(py, "encode", py_fn!(py, encode(value: u64)))?;
    Ok(m)
}

fn decodeat(py: Python, data: PyBytes, offset: usize) -> PyResult<(u64, usize)> {
    let (value, len) = data
        .data(py)
        .read_vlq_at(offset)
        .map_pyerr::<exc::ValueError>(py)?;
    Ok((value, len))
}

fn decode(py: Python, data: PyBytes) -> PyResult<u64> {
    let value = data.data(py).read_vlq().map_pyerr::<exc::ValueError>(py)?;
    Ok(value)
}

fn encode(py: Python, value: u64) -> PyResult<PyBytes> {
    let mut buf = Vec::new();
    buf.write_vlq(value).unwrap();
    Ok(PyBytes::new(py, &buf))
}
