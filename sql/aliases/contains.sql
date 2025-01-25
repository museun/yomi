select
    1
from
    aliases
where
    command = ?
    or alias = ?
limit
    1;
