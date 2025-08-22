/// Errors thrown by the [Source].
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
	#[error("Source internal error: {0}")]
	Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// [Diff] is a trait that contains information as to the diff between two [Content] types.
/// 
/// TODO: we need to standardize a diff output format, s.t., we can add a method to the [Source] trait.
pub trait Diff {
    
}


/// [Content] is a marker trait.
/// 
/// TODO: we should constrain this to have some kind of formatter.
pub trait Content {

}

/// [Referenced] marks the [Content] type that was originally referenced by the [Source].
pub trait Referenced: Content {
    
}

/// [Current] marks the [Content] type that is currently available via the [Source].
/// 
/// It should be able to able to [Diff] against a [Referenced] type.
pub trait Current<R: Referenced, D: Diff>: Content {
    fn diff(&self, other: &R) -> Result<D, SourceError>;
}

/// [Source] is a trait that allows for the creation of a [Content] type.
pub trait Source<R: Referenced, C: Current<R, D>, D: Diff> {
    fn get(&self) -> Result<Comparison<R, C, D>, SourceError>;
}

/// [Comparison] is the result of getting a source. 
pub struct Comparison<R: Referenced, C: Current<R, D>, D: Diff> {
    pub referenced: R,
    pub current: C,
    pub diff: D,
}

impl <R, C, D> Comparison<R, C, D> where R: Referenced, C: Current<R, D>, D: Diff {
    pub fn new(referenced: R, current: C, diff: D) -> Self {
        Self { referenced, current, diff }
    }

    pub fn referenced(&self) -> &R {
        &self.referenced
    }

    pub fn current(&self) -> &C {
        &self.current
    }

    pub fn diff(&self) -> &D {
        &self.diff
    }
}