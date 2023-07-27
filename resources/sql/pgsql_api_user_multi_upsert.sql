insert into portal.tbl_int_users (user_id, first_name, last_name, email, mod_de)
select a.user_id, a.first_name, a.last_name, a.email, a.mod_de
from jsonb_to_recordset($1::jsob) as a (user_id text, first_name text, last_name text, email text, mod_de text)
on conflict (user_id) do update set
    first_name = excluded.first_name,
    last_name = excluded.last_name,
    email = excluded.email,
    mod_de = excluded.mod_de,
    mod_timp = current_timestamp;
