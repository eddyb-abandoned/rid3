use gfx::Color;

macro_rules! expr { ($x:expr) => ($x) } // HACK work around macro limitations

pub struct SchemeIdx(usize);

macro_rules! __scheme {
    ($x:expr;) => {};
    ($x:expr; $scheme:ident, $($rest:ident,)*) => {
        #[allow(non_upper_case_globals)]
        pub const $scheme: SchemeIdx = SchemeIdx($x);
        __scheme!($x+1; $($rest,)*);
    }
}

macro_rules! schemes {
    ($($scheme:ident)|+; $($name:ident|$($r:tt, $g:tt, $b:tt)|*;)+) => {
        pub trait Scheme {
            $(fn $name(&self) -> Color;)*
        }
        impl Scheme for SchemeIdx {
            $(fn $name(&self) -> Color { expr!{
                [$([$r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0, 1.0]),*][self.0]
            } })*
        }
        __scheme!(0; $($scheme,)*);
    }
}

schemes! {
                Breeze     |BreezeLight|BreezeDark;
    // Window and Button
    back_alt   |189,195,199|224,223,222|77,77,77;
    background |239,240,241|239,240,241|49,54,59;
    focus      |61,174,233 |61,174,233 |61,174,233;
    hover      |147,206,233|142,203,233|61,174,233;
    active     |61,174,233 |255,128,224|61,174,233;
    inactive   |127,140,141|136,135,134|189,195,199;
    link       |41,128,185 |0,87,174   |41,128,185;
    negative   |218,68,83  |191,3,3    |218,68,83;
    neutral    |246,116,0  |176,128,0  |246,116,0;
    normal     |49,54,59   |49,54,59   |239,240,241;
    positive   |39,174,96  |0,110,40   |39,174,96;
    visited    |127,140,141|69,40,134  |127,140,141;
    // View
    back_view  |252,252,252|252,252,252|35,38,41;
    back_view_alt|239,240,241|248,247,246|49,54,59;
}
