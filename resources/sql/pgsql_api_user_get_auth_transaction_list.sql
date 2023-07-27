select distinct
    a.user_id,
    c.app_code,
    c.method_code
from portal.tbl_int_user_roles as a

inner join portal.tbl_int_user_authorization as b
on a.group_id = b.group_id

inner join portal.tbl_int_app_transactions as c
on b.app_method_id = c.id

where a.user_id = $1;