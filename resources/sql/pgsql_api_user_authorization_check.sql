select exists (
        select
            *
        from portal.tbl_int_user_roles as a

        inner join portal.tbl_int_user_authorization as b
        on a.group_id = b.group_id

        inner join portal.tbl_int_app_transactions as c
        on b.app_method_id = c.id

        where a.user_id = $1 and c.app_code = $2 and c.method_code = $3
    ) as rezult;
