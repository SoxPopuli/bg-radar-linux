pub mod alignment;
pub mod classes;
pub mod enemy_ally;
pub mod gender;
pub mod general;
pub mod race;
pub mod effect;

#[macro_export]
macro_rules! int_enum {
    ($viz: vis enum $name: ident : $repr: ty { $($k: ident = $v: expr),+ $(,)? }) => {
        #[allow(clippy::enum_variant_names)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr($repr)]
        $viz enum $name {
            $($k = $v),+
        }

        impl TryFrom<$repr> for $name {
            type Error = $crate::error::Error;

            fn try_from(value: $repr) -> Result<$name, Self::Error> {
                use $name::*;
                use $crate::error::Error::*;
                match value {
                    $($v => Ok($k)),+,
                    x => Err(InvalidEnumValue{ enum_type: stringify!($name), value: x.to_string() }),
                }
            }
        }
    };
}
