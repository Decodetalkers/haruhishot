#[derive(Debug, Default)]
pub struct Size<T = i32>
where
    T: Default,
{
    pub width: T,
    pub height: T,
}

#[derive(Debug, Default)]
pub struct Position<T = i32>
where
    T: Default,
{
    pub x: T,
    pub y: T,
}
