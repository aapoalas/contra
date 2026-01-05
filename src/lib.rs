use std::marker::PhantomData;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Invariant<'a>(PhantomData<fn(&'a ()) -> &'a ()>);
impl Invariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Contravariant<'a>(PhantomData<fn(&'a ())>);
impl Contravariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl Default for Contravariant<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct Arena(Vec<f64>);

#[derive(Debug, Clone, Copy)]
struct Handle<'a>(u32, Contravariant<'a>);

impl<'a> Handle<'a> {
    fn new(arena: &mut Arena, value: f64, _gc: NoGc<'a>) -> Handle<'a> {
        let index = u32::try_from(arena.0.len()).unwrap();
        arena.0.push(value);
        Handle(index, Contravariant::new())
    }

    fn get(self, arena: &Arena) -> &f64 {
        arena.0.get(self.0 as usize).as_ref().unwrap()
    }
}

#[derive(Debug)]
struct Gc<'a>(Contravariant<'a>);

impl<'gc> Gc<'gc> {
    fn nogc(&self) -> NoGc<'_> {
        NoGc(Invariant::new())
    }
}

#[derive(Debug, Clone, Copy)]
struct NoGc<'a>(Invariant<'a>);

impl<'a> From<Gc<'a>> for NoGc<'a> {
    fn from(_: Gc<'a>) -> Self {
        NoGc(Invariant::new())
    }
}

fn perform_gc<'a>(arena: &mut Arena, gc: Gc<'a>) -> Gc<'a> {
    arena.0.clear();
    gc
}

fn outer<'gc: 'handle, 'handle>(
    arena: &mut Arena,
    handle: Handle<'handle>,
    mut gc: Gc<'gc>,
) -> Handle<'handle> {
    gc = perform_gc(arena, gc);
    // The following line must fail: perform_gc must invalidate handle.
    handle.get(arena);
    Handle::new(arena, 3.0, gc.nogc())
}

pub fn main() {
    let mut arena = Arena(Vec::with_capacity(1024));
    let gc = Gc(Contravariant::new());
    let handle = Handle::new(&mut arena, 1.0, gc.nogc());
    outer(&mut arena, handle, gc);
}
