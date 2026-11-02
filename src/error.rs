#[macro_export]
macro_rules! declare_error {
    (@name $name:ident $($rest:tt)*) => { $crate::declare_error!(@impl $name $($rest)*); };
    (@impl $name:ident
        $(@derive[$($derive:path),* $(,)?])?
        $(
        $(#[doc = $group_doc:literal])*
        $group:ident {
            $(
                $(#[$attr:meta])*
                $variant:ident
                    $( { $($field:ident : $fty:ty),* $(,)? } )?
                    $( ( $($ty:ty),* $(,)? ) )?
                    => $message:literal
            ),* $(,)?
        }
    )*) => {
        #[derive(Debug, thiserror::Error $(, $($derive),*)?)]
        pub enum $name {
            $($(
                $(#[$attr])*
                #[error($message)]
                $variant
                    $( { $($field : $fty),* } )?
                    $( ( $($ty),* ) )?,
            )*)*
        }
    };
    ($($rest:tt)*) => { $crate::declare_error!(@impl Error $($rest)*); };
}
