//! serde-saphyr errors as a line and a message for people.

use serde_saphyr::{MessageFormatter, UserMessageFormatter};

/// The error's line inside the parsed text, counting from 1.
pub fn error_line(error: &serde_saphyr::Error) -> Option<u32> {
    let line = error.location()?.line();
    u32::try_from(line).ok().filter(|line| *line > 0)
}

// The user formatter leaves out the location, which callers report as a line, and the
// library's own setting names.
pub fn error_message(error: &serde_saphyr::Error) -> String {
    UserMessageFormatter
        .format_message(error.without_snippet())
        .into_owned()
}
