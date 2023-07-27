select
    *
from portal.tbl_int_app_transactions as a
where a.app_code = $1;