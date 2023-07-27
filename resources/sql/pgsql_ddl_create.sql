do $$
begin
    raise notice 'CREATING EXTENSION "uuid-ossp"';
    create extension if not exists "uuid-ossp";

    raise notice 'CREATING SCHEMA "portal"';
    create schema if not exists portal;

    /* 0001.001 */
    raise notice 'CREATING TABLE "tbl_int_users"';
    create table if not exists portal.tbl_int_users (
        user_id text not null,
        first_name text not null,
        last_name text not null,
        email text not null,
        mod_de text not null,
        mod_timp timestamp not null default current_timestamp,
        constraint tbl_int_users_pk primary key (user_id),
        constraint tbl_int_users_ck1 check (email ~* '^[a-zA-Z0-9_+&-]+(?:.[a-zA-Z0-9_+&-]+)*@(?:[a-zA-Z0-9-]+.)+[a-zA-Z]{2,7}$')
    );

    /* 0001.002 */
    raise notice 'CREATING TABLE "tbl_int_user_groups"';
    create table if not exists portal.tbl_int_user_groups (
        group_id text not null,
        group_name text not null,
        mod_de text not null,
        mod_timp timestamp not null default current_timestamp,
        constraint tbl_int_user_groups_pk primary key (group_id)
    );

    /* 0001.003 */
    raise notice 'CREATING TABLE "tbl_int_user_roles"';
    create table if not exists portal.tbl_int_user_roles (
        id uuid not null default uuid_generate_v4(),
        user_id text not null,
        group_id text not null,
        mod_de text not null,
        mod_timp timestamp not null default current_timestamp,
        constraint tbl_int_user_roles_pk primary key (id),
        constraint tbl_int_user_roles_fk_user_id foreign key (user_id) references portal.tbl_int_users (user_id),
        constraint tbl_int_user_roles_fk_group_id foreign key (group_id) references portal.tbl_int_user_groups (group_id),
        constraint tbl_int_user_roles_unique_1 unique (user_id, group_id)
    );

    /* 0001.004 */
    raise notice 'CREATING TABLE "tbl_int_app_transactions"';
    create table if not exists portal.tbl_int_app_transactions (
        id uuid not null default uuid_generate_v4(),
        app_code text not null,
        method_code text not null,
        descr text not null,
        mod_de text not null,
        mod_timp timestamp not null default current_timestamp,
        constraint tbl_int_app_transactions_pk primary key (id),
        constraint tbl_int_app_transactions_unique_1 unique (app_code, method_code)
    );

    insert into portal.tbl_int_app_transactions (app_code, method_code, descr, mod_de)
    values ('portal', 'user_single_get', 'Get data for one app user by user id', 'catalin'),
        ('portal', 'user_single_persist', 'Add/ update data for one app user', 'catalin'),
        ('portal', 'user_single_delete', 'Delete data for one app user', 'catalin'),
        ('portal', 'user_all_list', 'Get data for all app users', 'catalin'),
        ('portal', 'app_method_app_codes', 'TBD', 'catalin'),
        ('portal', 'app_method_list_all', 'TBD', 'catalin'),
        ('portal', 'app_method_get_single_by_id', 'TBD', 'catalin'),
        ('portal', 'app_method_del_single_by_id', 'TBD', 'catalin'),
        ('portal', 'app_method_upsert_single', 'TBD', 'catalin'),
        ('portal', 'app_method_upsert_all', 'TBD', 'catalin')
    on conflict (app_code, method_code) do nothing;

    /* 0001.005 */
    raise notice 'CREATING TABLE "tbl_int_user_authorization"';
    create table if not exists portal.tbl_int_user_authorization (
        id uuid not null default uuid_generate_v4(),
        group_id text not null,
        app_method_id uuid not null,
        mod_de text not null,
        mod_timp timestamp not null default current_timestamp,
        constraint tbl_int_user_authorization_pk primary key (id),
        constraint tbl_int_user_authorization_fk_group_id foreign key (group_id) references portal.tbl_int_user_groups (group_id),
        constraint tbl_int_user_authorization_fk_app_method_id foreign key (app_method_id) references portal.tbl_int_app_transactions (id),
        constraint tbl_int_user_authorization_unique_1 unique (group_id, app_method_id)
    );

    insert into portal.tbl_int_user_authorization (group_id, app_method_id, mod_de)
    values ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'user_single_get'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'user_single_persist'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'user_single_delete'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'user_all_list'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'app_method_app_codes'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'app_method_list_all'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'app_method_get_single_by_id'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'app_method_del_single_by_id'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'app_method_upsert_single'), 'catalin'),
        ('cdg_admin', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'app_method_upsert_all'), 'catalin'),
        ('cdg_controller', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'user_single_get'), 'catalin'),
        ('cdg_controller', (select id from portal.tbl_int_app_transactions where app_code = 'portal' and method_code = 'user_all_list'), 'catalin')
    on conflict (group_id, app_method_id) do nothing;

    /* 0001.006 */
    raise notice 'CREATING TABLE "tbl_int_user_authentication"';
    create table if not exists portal.tbl_int_user_authentication (
        id uuid not null default uuid_generate_v4(),
        user_id text not null,
        token_id uuid not null,
        mod_timp timestamp not null,
        constraint tbl_int_user_authentication_pk primary key (id),
        constraint tbl_int_user_authentication_fk_user_id foreign key (user_id) references portal.tbl_int_users (user_id),
        constraint tbl_int_user_authentication_uq_user_id unique (user_id)
    );
end;
$$ language plpgsql;