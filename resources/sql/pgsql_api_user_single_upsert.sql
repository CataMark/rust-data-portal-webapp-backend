insert into portal.tbl_int_users (user_id, first_name, last_name, email, mod_de)
values ($1, $2, $3, $4, $5)
on conflict (user_id) do update set
    first_name = excluded.first_name,
    last_name = excluded.last_name,
    email = excluded.email,
    mod_de = excluded.mod_de,
    mod_timp = current_timestamp
returning *;