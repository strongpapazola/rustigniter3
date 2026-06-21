//! Macro pembantu untuk mengurangi boilerplate.

/// Bangkitkan implementasi [`Controller`](crate::system::Controller) yang memetakan nama
/// aksi (segmen method pada URL) ke method dengan nama sama. Setiap method harus bertanda
/// tangan `fn(&self, &mut Ctx) -> Response`. Aksi tak dikenal → 404.
///
/// Menggantikan boilerplate `match action { ... }`:
///
/// ```ignore
/// struct Welcome;
/// impl Welcome {
///     fn index(&self, ctx: &mut Ctx) -> Response { ctx.view("welcome_message", json!({})) }
/// }
/// crate::actions!(Welcome { index });
/// ```
#[macro_export]
macro_rules! actions {
    ($ctrl:ty { $($action:ident),* $(,)? }) => {
        impl $crate::system::Controller for $ctrl {
            fn dispatch(
                &self,
                __action: &str,
                __ctx: &mut $crate::system::Ctx,
            ) -> ::std::option::Option<$crate::system::Response> {
                match __action {
                    $( stringify!($action) => ::std::option::Option::Some(self.$action(__ctx)), )*
                    _ => ::std::option::Option::None,
                }
            }
        }
    };
}
