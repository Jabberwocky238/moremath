pub trait FromWith<T, C>: Sized {
    fn from_with(value: T, context: C) -> Self;
}

pub trait IntoWith<T, C>: Sized {
    fn into_with(self, context: C) -> T;
}

impl<T, U, C> IntoWith<U, C> for T
where
    U: FromWith<T, C>,
{
    fn into_with(self, context: C) -> U {
        U::from_with(self, context)
    }
}

pub trait TryFromWith<T, C>: Sized {
    type Error;

    fn try_from_with(value: T, context: C) -> Result<Self, Self::Error>;
}

pub trait TryIntoWith<T, C>: Sized {
    type Error;

    fn try_into_with(self, context: C) -> Result<T, Self::Error>;
}

impl<T, U, C> TryIntoWith<U, C> for T
where
    U: TryFromWith<T, C>,
{
    type Error = U::Error;

    fn try_into_with(self, context: C) -> Result<U, Self::Error> {
        U::try_from_with(self, context)
    }
}

#[allow(dead_code)]
#[repr(C)]
struct ConflictWitness;
