use crate::span::Span;

#[derive(Debug)]
pub struct ErrorAggregate<T: std::error::Error>(Vec<ErrorBy<T>>);

impl<T: std::error::Error> ErrorAggregate<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn resolve(self) -> Result<(), Self> {
        match self.0.is_empty() {
            true => Ok(()),
            false => Err(self),
        }
    }

    pub fn add_error(&mut self, error_type: T) {
        self.0.push(ErrorBy::new(error_type));
    }
}

impl<T> std::fmt::Display for ErrorAggregate<T>
where
    T: std::error::Error,
    ErrorBy<T>: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, error) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }

        Ok(())
    }
}

impl<T> std::error::Error for ErrorAggregate<T>
where
    T: std::error::Error,
    ErrorBy<T>: std::fmt::Display,
{
}

#[derive(thiserror::Error, Clone, Debug, PartialEq)]
#[error("{error_type} at {span:?}")]
pub struct ErrorBy<T: std::error::Error> {
    error_type: T,
    span: Option<Span>,
}

impl<T: std::error::Error> ErrorBy<T> {
    pub fn new(error_type: T) -> Self {
        Self {
            error_type,
            span: None,
        }
    }
}
