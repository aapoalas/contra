use std::marker::PhantomData;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Covariant<'a>(PhantomData<fn() -> &'a ()>);
impl Covariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}
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
        Handle(index, PhantomData)
    }

    fn get(self, arena: &Arena) -> &f64 {
        arena.0.get(self.0 as usize).as_ref().unwrap()
    }

    fn add(self, arena: &mut Arena, b: Self, gc: NoGc<'a>) -> Handle<'a> {
        Handle::new(arena, self.get(arena) + b.get(arena), gc)
    }

    fn sub(self, arena: &mut Arena, b: Self, gc: NoGc<'a>) -> Handle<'a> {
        Handle::new(arena, self.get(arena) - b.get(arena), gc)
    }
}

impl<'a> Handle<'a> {
    fn bind(self, _gc: NoGc<'a>) -> Self {
        Handle(self.0, Contravariant::new())
    }

    fn unbind(self) -> Handle<'static> {
        Handle(self.0, Contravariant::new())
    }
}

#[derive(Debug)]
struct Gc<'a>(Invariant<'a>);

impl<'gc> Gc<'gc> {
    fn nogc(&self) -> NoGc<'_> {
        NoGc(Invariant::new())
    }

    // fn reborrow(&mut self) -> Gc<'_> {
    //     Gc(Invariant::new())
    // }

    fn bind_one(&mut self, a: Handle<'gc>) -> Gc<'_> {
        Gc(Invariant::new())
    }

    fn bind_two(&mut self, a: Handle<'gc>, b: Handle<'gc>) -> Gc<'_> {
        Gc(Invariant::new())
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
    println!("Do GC");
    arena.0.clear();
    gc
}

fn try_foo<'gc>(arena: &mut Arena, handle: Handle<'gc>, gc: NoGc<'gc>) -> Option<Handle<'gc>> {
    None
}

fn do_foo<'gc>(arena: &mut Arena, handle: Handle<'gc>, gc: Gc<'gc>) -> (Handle<'gc>, Gc<'gc>) {
    let data = handle.get(arena).abs();
    let gc = perform_gc(arena, gc);
    (Handle::new(arena, data, gc.nogc()), gc)
}

fn inner_b<'gc>(
    arena: &mut Arena,
    a: Handle<'gc>,
    b: Handle<'gc>,
    mut gc: Gc<'gc>,
) -> (Handle<'gc>, Gc<'gc>) {
    let nogc = gc.nogc();
    let a = a.bind(nogc);
    let b = b.bind(nogc);
    let mut c = Handle::new(arena, 0.0124, nogc);
    // let c = a.sub(arena, b, nogc);
    // Try commenting out this line...
    let a = *a.get(arena);
    let scoped_c = *c.get(arena);
    gc = perform_gc(arena, gc);
    let nogc = gc.nogc();
    // ... and this line, and compare to do_foo below.
    let a = Handle::new(arena, a, nogc);
    let mut c = Handle::new(arena, scoped_c, nogc);
    let d = if let Some(d) = try_foo(arena, a, nogc) {
        d
    } else {
        // This should fail from c having shared access to gc while we take
        // exclusive access to it here. We would have to repeat scoped_c
        // work here and replace
        let scoped_c = *c.get(arena);
        let (d, gcc) = do_foo(arena, a, gc);
        gc = gcc;
        c = Handle::new(arena, scoped_c, gc.nogc());
        d
    };
    // c has been garbage collected here and no longer exists: panics.
    (d.sub(arena, c, gc.nogc()), gc)
}

fn inner_a<'gc>(arena: &mut Arena, a: Handle, b: Handle, gc: Gc<'gc>) -> Handle<'gc> {
    let nogc = gc.nogc();
    let a = a.bind(nogc);
    let b = b.bind(nogc);
    let c = a.add(arena, b, nogc);
    inner_b(arena, a, c, gc)
}

fn main() {
    let mut arena = Arena(Vec::with_capacity(1024));
    let gc = Gc(Contravariant::new());
    let a = Handle::new(&mut arena, 0.0, gc.nogc());
    let b = Handle::new(&mut arena, 1.0, gc.nogc());
    inner_a(&mut arena, a, b, gc);
}
