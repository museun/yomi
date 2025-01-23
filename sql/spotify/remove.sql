delete from kv
where
    id = (
        select
            id
        from
            kv
        where
            key = ?
        order by
            id desc
        limit
            1
    );
