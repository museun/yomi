select
    key,
    value
from
    kv
order by
    id desc
limit
    ?;
