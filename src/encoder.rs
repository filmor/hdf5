use rustc_serialize;
use std::mem;

use data::{self, Array, Compound, Data};
use datatype::{self, Datatype};
use file::File;
use {Error, Result};

/// An encoder.
pub struct Encoder<'l> {
    file: &'l File,
    name: Option<String>,
    state: State,
}

enum State {
    Uncertain,
    Sequence(Sequence),
    Structure(Structure),
}

struct Sequence {
    data: Vec<u8>,
    datatype: Option<Datatype>,
}

struct Structure {
    data: Vec<u8>,
    fields: Vec<(String, Datatype, usize)>,
}

impl<'l> Encoder<'l> {
    /// Create an encoder.
    pub fn new(file: &'l File, name: &str) -> Result<Encoder<'l>> {
        Ok(Encoder { file: file, name: Some(name.to_string()), state: State::Uncertain })
    }

    fn element<T: Data>(&mut self, data: T) -> Result<()> {
        match self.state {
            State::Uncertain => match self.name.take() {
                Some(ref name) => self.file.write(name, data),
                _ => raise!("cannot write data without a name"),
            },
            State::Sequence(ref mut sequence) => {
                if let Some(ref datatype) = sequence.datatype {
                    if datatype != &data.datatype() {
                        raise!("cannot mix datatypes in arrays");
                    }
                } else {
                    sequence.datatype = Some(data.datatype());
                }
                sequence.data.extend(data.as_bytes());
                Ok(())
            },
            State::Structure(ref mut structure) => match self.name.take() {
                Some(name) => {
                    println!("Field: {}", &name);
                    let size = structure.data.len();
                    structure.data.extend(data.as_bytes());
                    let size = structure.data.len() - size;
                    structure.fields.push((name, data.datatype(), size));
                    Ok(())
                },
                _ => raise!("cannot write a field without a name"),
            },
        }
    }

    fn sequence<F>(&mut self, next: F) -> Result<()> where F: FnOnce(&mut Self) -> Result<()> {
        let state = mem::replace(&mut self.state, State::Sequence(Sequence::new()));
        try!(next(self));
        match mem::replace(&mut self.state, state) {
            State::Sequence(sequence) => match self.name.take() {
                Some(ref name) => self.file.write(name, try!(sequence.coagulate())),
                _ => raise!("cannot write an array without a name"),
            },
            _ => unreachable!(),
        }
    }

    fn structure<F>(&mut self, next: F) -> Result<()> where F: FnOnce(&mut Self) -> Result<()> {
        let state = mem::replace(&mut self.state, State::Structure(Structure::new()));
        try!(next(self));
        match mem::replace(&mut self.state, state) {
            State::Structure(structure) => match self.name.take() {
                Some(ref name) => self.file.write(name, try!(structure.coagulate())),
                _ => raise!("cannot write a struct without a name"),
            },
            _ => unreachable!(),
        }
    }

    fn structure_field<F>(&mut self, name: &str, next: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        let name = mem::replace(&mut self.name, Some(name.to_string()));
        try!(next(self));
        match mem::replace(&mut self.name, name) {
            None => Ok(()),
            _ => raise!("found a field without a value"),
        }
    }
}

impl Sequence {
    #[inline]
    fn new() -> Sequence {
        Sequence { data: vec![], datatype: None }
    }

    fn coagulate(self) -> Result<Array<u8>> {
        let Sequence { data, datatype } = self;
        let datatype = match datatype {
            Some(datatype) => {
                let size = try!(datatype.size());
                if data.len() % size != 0 {
                    raise!("encountered malformed array data");
                }
                try!(datatype::new_array(datatype, &[1, data.len() / size]))
            },
            _ => raise!("cannot infer the datatype of empty arrays"),
        };
        data::new_array(data, datatype)
    }
}

impl Structure {
    fn new() -> Structure {
        Structure { data: vec![], fields: vec![] }
    }

    fn coagulate(self) -> Result<Compound> {
        let Structure { data, fields } = self;
        let datatype = try!(datatype::new_compound(&fields));
        data::new_compound(data, datatype)
    }
}

impl<'l> rustc_serialize::Encoder for Encoder<'l> {
    type Error = Error;

    fn emit_bool(&mut self, _: bool) -> Result<()> {
        panic!("HDF5 does not support booleans");
    }

    #[inline]
    fn emit_char(&mut self, value: char) -> Result<()> {
        self.element(value as u32)
    }

    fn emit_enum<F>(&mut self, _: &str, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        unimplemented!();
    }

    fn emit_enum_struct_variant<F>(&mut self, _: &str, _: usize, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support enum structs");
    }

    fn emit_enum_struct_variant_field<F>(&mut self, _: &str, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support enum structs");
    }

    fn emit_enum_variant<F>(&mut self, _: &str, _: usize, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        unimplemented!();
    }

    fn emit_enum_variant_arg<F>(&mut self, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support enums with arguments");
    }

    #[inline]
    fn emit_f32(&mut self, value: f32) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_f64(&mut self, value: f64) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_i8(&mut self, value: i8) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_i16(&mut self, value: i16) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_i32(&mut self, value: i32) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_i64(&mut self, value: i64) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_u64(&mut self, value: u64) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_isize(&mut self, value: isize) -> Result<()> {
        self.element(value)
    }

    fn emit_map<F>(&mut self, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support maps");
    }

    fn emit_map_elt_key<F>(&mut self, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support maps");
    }

    fn emit_map_elt_val<F>(&mut self, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support maps");
    }

    #[inline]
    fn emit_nil(&mut self) -> Result<()> {
        panic!("HDF5 does not support nils");
    }

    fn emit_option<F>(&mut self, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support options");
    }

    fn emit_option_none(&mut self) -> Result<()> {
        panic!("HDF5 does not support options");
    }

    fn emit_option_some<F>(&mut self, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support options");
    }

    fn emit_seq<F>(&mut self, _: usize, next: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        self.sequence(next)
    }

    fn emit_seq_elt<F>(&mut self, _: usize, next: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        next(self)
    }

    fn emit_str(&mut self, value: &str) -> Result<()> {
        self.element(value)
    }

    fn emit_struct<F>(&mut self, _: &str, _: usize, next: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        self.structure(next)
    }

    fn emit_struct_field<F>(&mut self, name: &str, _: usize, next: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        self.structure_field(name, next)
    }

    fn emit_tuple<F>(&mut self, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support tuples");
    }

    fn emit_tuple_arg<F>(&mut self, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support tuples");
    }

    fn emit_tuple_struct<F>(&mut self, _: &str, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support tuple structs");
    }

    fn emit_tuple_struct_arg<F>(&mut self, _: usize, _: F) -> Result<()>
        where F: FnOnce(&mut Self) -> Result<()>
    {
        panic!("HDF5 does not support tuple structs");
    }

    #[inline]
    fn emit_u8(&mut self, value: u8) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_u16(&mut self, value: u16) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_u32(&mut self, value: u32) -> Result<()> {
        self.element(value)
    }

    #[inline]
    fn emit_usize(&mut self, value: usize) -> Result<()> {
        self.element(value)
    }
}
