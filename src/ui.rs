use eyre::eyre;
use eyre::Result;
use gtk4::{glib::object::IsA, AlertDialog, Builder};

pub fn get_object<T>(builder: &Builder, name: &str) -> Result<T>
where
    T: IsA<gtk4::glib::Object>,
{
    builder.object(name).ok_or(eyre!(
        "Unable to get UI element {}, this likely means the XML was changed/corrupted.",
        name
    ))
}

pub fn build_fail_alert() -> AlertDialog {
    AlertDialog::builder()
        .message("Authorization failed for some reason. Check your login details and try again.")
        .buttons(vec!["Ok"])
        .build()
}
