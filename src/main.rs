pub mod french;
pub mod game;

fn main() {
    //game::euchre::cli_main();
    let tui = game::euchre::Tui::default();
    let terminal = game::euchre::tui_init().unwrap();
    tui.run(terminal).unwrap();
    game::euchre::tui_restore().unwrap();
}
