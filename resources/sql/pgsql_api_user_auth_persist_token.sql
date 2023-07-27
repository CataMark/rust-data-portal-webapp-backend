insert into portal.tbl_int_user_authentication (user_id, token_id, mod_timp)
values ($1, $2, $3)
on conflict (user_id) do update set
    token_id = excluded.token_id,
    mod_timp = excluded.mod_timp
returning *;