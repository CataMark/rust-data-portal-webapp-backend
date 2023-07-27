insert into portal.tbl_int_app_transactions (app_code, method_code, descr, mod_de)
values ($1, $2, $3, $4)
on conflict (app_code, method_code) do update set
    descr = excluded.descr,
    mod_de = excluded.mod_de,
    mod_timp = current_timestamp
returning *;