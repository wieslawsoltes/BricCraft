use crate::runtime::Runtime;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{
    Application, ApplicationImpl, ApplicationImplExt, NewApplication, TopLevel,
};
use ferroui_view::{ComponentHost, Owner};
use std::rc::Rc;

#[repr(C)]
pub struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_name(Some("BricCraft".into()));
        this.set_requested_theme_variant(Some(ThemeVariant::dark()));
        this.styles()
            .add(ferroui_themes_simple::SimpleTheme::new().as_style());
    }
    fn on_framework_initialization_completed(this: &Self) {
        let owner = Owner::new_root();
        let runtime = owner.with(|| Rc::new(Runtime::new(owner)));
        let host = ComponentHost::with_view({
            let runtime = runtime.clone();
            move || runtime.view()
        });
        // The world is full bleed; the declarative interface layer applies the
        // framework's safe area without shrinking the camera's render target.
        TopLevel::set_auto_safe_area_padding(&host, false);
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            lifetime.set_main_view(Some(host.upcast()));
        }
        crate::bridge::RUNTIME.with(|slot| *slot.borrow_mut() = Some(runtime));
        Self::parent_on_framework_initialization_completed(this);
    }
}
