use futures03::Stream;
use std::fmt;

#[derive(Debug, Clone)]
pub struct FirehoseCursor(Option<String>);

impl FirehoseCursor {
    #[allow(non_upper_case_globals)]
    pub const None: Self = FirehoseCursor(None);

    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

impl fmt::Display for FirehoseCursor {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(self.0.as_deref().unwrap_or(""))
    }
}

impl From<String> for FirehoseCursor {
    fn from(cursor: String) -> Self {
        // Treat a cursor of "" as None, not absolutely necessary for correctness since the firehose
        // treats both as the same, but makes it a little clearer.
        if cursor.is_empty() {
            FirehoseCursor::None
        } else {
            FirehoseCursor(Some(cursor))
        }
    }
}

impl From<Option<String>> for FirehoseCursor {
    fn from(cursor: Option<String>) -> Self {
        match cursor {
            None => FirehoseCursor::None,
            Some(s) => FirehoseCursor::from(s),
        }
    }
}

impl AsRef<Option<String>> for FirehoseCursor {
    fn as_ref(&self) -> &Option<String> {
        &self.0
    }
}
