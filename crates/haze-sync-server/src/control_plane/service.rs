//! Server-owned Stage 11 control-plane service.

include!("service/prelude.rs");

impl ControlPlaneServices {
    include!("service/methods_00.rs");
    include!("service/methods_01.rs");
    include!("service/methods_02.rs");
    include!("service/methods_03.rs");
    include!("service/methods_04.rs");
    include!("service/methods_05.rs");
    include!("service/methods_06.rs");
}

include!("service/postlude.rs");
