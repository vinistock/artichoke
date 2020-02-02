use std::str;

use crate::convert::{ConvertMut, TryConvert};
use crate::types::Rust;
use crate::value::Value;
use crate::{Artichoke, ArtichokeError};

impl ConvertMut<String, Value> for Artichoke {
    fn convert_mut(&mut self, value: String) -> Value {
        // Ruby `String`s are just bytes, so get a pointer to the underlying
        // `&[u8]` infallibly and convert that to a `Value`.
        self.convert_mut(value.as_bytes())
    }
}

impl ConvertMut<&str, Value> for Artichoke {
    fn convert_mut(&mut self, value: &str) -> Value {
        // Ruby `String`s are just bytes, so get a pointer to the underlying
        // `&[u8]` infallibly and convert that to a `Value`.
        self.convert_mut(value.as_bytes())
    }
}

impl TryConvert<Value, String> for Artichoke {
    fn try_convert(&self, value: Value) -> Result<String, ArtichokeError> {
        TryConvert::<_, &str>::try_convert(self, value).map(String::from)
    }
}

impl<'a> TryConvert<Value, &'a str> for Artichoke {
    fn try_convert(&self, value: Value) -> Result<&'a str, ArtichokeError> {
        let type_tag = value.ruby_type();
        self.try_convert(value)
            .ok()
            .and_then(|bytes| {
                // This converter requires that the bytes be valid UTF-8 data.
                // If the `Value` contains binary data, use the `Vec<u8>` or
                // `&[u8]` converter.
                str::from_utf8(bytes).ok()
            })
            .ok_or(ArtichokeError::ConvertToRust {
                from: type_tag,
                to: Rust::String,
            })
    }
}

#[cfg(test)]
// Convert<String> is implemented in terms of Convert<&str> so only implement
// the tests for String to exercise both code paths.
mod tests {
    use quickcheck_macros::quickcheck;
    use std::convert::TryFrom;

    use crate::test::prelude::*;

    #[test]
    fn fail_convert() {
        let mut interp = crate::interpreter().expect("init");
        // get a mrb_value that can't be converted to a primitive type.
        let value = interp.eval(b"Object.new").expect("eval");
        let expected = Err(ArtichokeError::ConvertToRust {
            from: Ruby::Object,
            to: Rust::String,
        });
        let result = value.try_into::<String>();
        assert_eq!(result, expected);
    }

    #[allow(clippy::needless_pass_by_value)]
    #[quickcheck]
    fn convert_to_string(s: String) -> bool {
        let mut interp = crate::interpreter().expect("init");
        let mrb = interp.0.borrow().mrb;
        let value = interp.convert_mut(s.clone());
        let ptr = unsafe { sys::mrb_string_value_ptr(mrb, value.inner()) };
        let len = unsafe { sys::mrb_string_value_len(mrb, value.inner()) };
        let string =
            unsafe { std::slice::from_raw_parts(ptr as *const u8, usize::try_from(len).unwrap()) };
        s.as_bytes() == string
    }

    #[allow(clippy::needless_pass_by_value)]
    #[quickcheck]
    fn string_with_value(s: String) -> bool {
        let mut interp = crate::interpreter().expect("init");
        let value = interp.convert_mut(s.clone());
        value.to_s() == s.as_bytes()
    }

    #[allow(clippy::needless_pass_by_value)]
    #[quickcheck]
    fn roundtrip(s: String) -> bool {
        let mut interp = crate::interpreter().expect("init");
        let value = interp.convert_mut(s.clone());
        let value = value.try_into::<String>().expect("convert");
        value == s
    }

    #[quickcheck]
    fn roundtrip_err(b: bool) -> bool {
        let interp = crate::interpreter().expect("init");
        let value = interp.convert(b);
        let value = value.try_into::<String>();
        let expected = Err(ArtichokeError::ConvertToRust {
            from: Ruby::Bool,
            to: Rust::String,
        });
        value == expected
    }

    #[test]
    fn symbol_to_string() {
        let mut interp = crate::interpreter().expect("init");
        let value = interp.eval(b":sym").expect("eval");
        let value = value.try_into::<String>().expect("convert");
        assert_eq!(&value, "sym");
    }
}
