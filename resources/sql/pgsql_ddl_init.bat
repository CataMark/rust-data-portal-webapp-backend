@echo off
setlocal

set PATH=%PATH%;C:\Program Files\PostgreSQL\14\bin
set PGPASSWORD=postgres
set PGUSER=postgres
set PGHOST=localhost
SET SQL_FILE=assets/sql/pgsql_ddl_create.sql

for /f "usebackq tokens=1" %%i in (`psql -U %PGUSER% -h %PGHOST% -c "select * from pg_database where datname = 'cdg'"`) do @set "DB_EXISTS=%%i"
if %DB_EXISTS%=="(0" (psql -U %PGUSER% -h %PGHOST% -c "create database cdg;")
psql -U %PGUSER% -h %PGHOST% -d cdg -f %SQL_FILE%

endlocal