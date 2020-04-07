use pancurses::{Input, Window};

// TODO: handle arrow keys
pub fn prompt(window: &Window, ps: Option<&str>, password: bool) -> Option<String> {
    if let Some(ps) = ps {
        window.mvaddstr(0, 0, ps);
        window.refresh();
    }
    if password {
        pancurses::noecho();
    }
    window.keypad(true);
    let mut output = String::new();
    loop {
        match window.getch() {
            Some(Input::Character('\n')) => break,
            Some(Input::Character(c)) => output.push(c),
            Some(Input::KeyEnter) => break,
            Some(Input::KeyBackspace) => {
                output.pop();
            }
            Some(Input::KeyCancel) => return None,
            _ => {}
        }
    }
    pancurses::echo();
    Some(output)
}
