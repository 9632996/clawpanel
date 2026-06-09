use serde::Serialize;
use serde_json::Value;

pub(crate) fn to_value_ref<T>(value: &T) -> Value
where
    T: Serialize + ?Sized,
{
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(_) => Value::Null,
    }
}

pub(crate) fn object_key(raw: &str) -> String {
    raw.strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(raw)
        .to_string()
}

#[macro_export]
macro_rules! jv {
    (null) => {
        serde_json::Value::Null
    };
    ([]) => {
        serde_json::Value::Array(Vec::new())
    };
    ([ $($element:tt),* $(,)? ]) => {
        serde_json::Value::Array(vec![$($crate::jv!($element)),*])
    };
    ({}) => {
        serde_json::Value::Object(serde_json::Map::new())
    };
    ({ $($key:tt : $value:tt),* $(,)? }) => {{
        let mut object = serde_json::Map::new();
        $(
            object.insert(
                $crate::json_value::object_key(stringify!($key)),
                $crate::jv!($value),
            );
        )*
        serde_json::Value::Object(object)
    }};
    ({ $($key:tt : $value:expr),* $(,)? }) => {{
        let mut object = serde_json::Map::new();
        $(
            object.insert(
                $crate::json_value::object_key(stringify!($key)),
                $crate::json_value::to_value_ref(&$value),
            );
        )*
        serde_json::Value::Object(object)
    }};
    ($other:expr) => {
        $crate::json_value::to_value_ref(&$other)
    };
}
