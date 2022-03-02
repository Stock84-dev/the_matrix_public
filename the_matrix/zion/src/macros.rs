#[macro_export]
macro_rules! read_all_loop {
    ($reader:ident) => {
        match $reader.try_read() {
            Some(x) => x,
            None => continue,
        }
        .read_all()
    };
}

#[macro_export]
macro_rules! read_all {
    ($reader:ident) => {
        match $reader.try_read() {
            Some(x) => x,
            None => return,
        }
        .read_all()
    };
}

#[macro_export]
macro_rules! read_loop {
    ($reader:ident) => {
        match $reader.try_read() {
            Some(x) => x,
            None => continue,
        }
        .read()
    };
}

#[macro_export]
macro_rules! read {
    ($reader:ident) => {
        match $reader.try_read() {
            Some(x) => x,
            None => return,
        }
        .read()
    };
}
