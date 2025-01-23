insert into
    kv (key, value)
select
    ?,
    ?
where
    (
        select
            key
        from
            kv
        order by
            id desc
        limit
            1
    ) is null
    or (
        select
            key
        from
            kv
        order by
            id desc
        limit
            1
    ) != ?;
