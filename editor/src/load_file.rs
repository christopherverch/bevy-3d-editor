use bevy::prelude::*;
use rfd::FileDialog;
pub fn open_file_dialog(keys: Res<ButtonInput<KeyCode>>, asset_server: Res<AssetServer>) {
    // Example: Open the dialog when the user presses "O"
    if keys.just_pressed(KeyCode::KeyO) {
        if let Some(path) = FileDialog::new().pick_file() {
            println!("Selected file: {:?}", path);
            let gltf: Handle<Gltf> = asset_server.load(path);
        } else {
            println!("No file selected");
        }
    }
}
