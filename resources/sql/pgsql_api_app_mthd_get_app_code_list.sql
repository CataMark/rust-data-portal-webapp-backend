select distinct
    a.app_code
from portal.tbl_int_app_transactions as a
order by a.app_code asc;