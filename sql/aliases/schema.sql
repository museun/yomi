PRAGMA foreign_keys = on;

create table
    if not exists commands (command text not null unique);

create table
    if not exists aliases (
        command text not null,
        alias text not null,
        foreign key (command) references commands (command) on delete cascade,
        unique (command, alias),
        unique (alias)
    );
