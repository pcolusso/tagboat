mod common;

use tagger::App;

#[test]
fn test_adding_files() {
    let mut app = App::init("test.db").unwrap();
    app.create_file(".").unwrap();
}
