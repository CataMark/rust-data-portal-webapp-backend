# Backend server for a data portal web app

## Features
- token based magic link login
- token based authentication/ authorisation
- REST API
- mail notifications
- db async queries and data upload/ download using .xlsx/ .csv/. txt/ .json
- endpoint authorisations based on user groups

## Dependecies
- actix
- tokio
- serde
- jwt
- openssl
- dbpool (my own lib)
- ...


## Prerequisites:

-   install **OpenSSL** library
-   generate cryptographic key pairs

## Backlog:

-   [ ] Application transactions api's

    -   [x] make object template
    -   [x] get list
    -   [x] download list to xlsx
    -   [x] add item
    -   [x] update item
    -   [x] delete item
    -   [x] upsert from xlsx
    -   [] upsert from csv

-   [ ] Request Form Multipart - extractor

    -   [x] extract binary files
    -   [ ] extract text files
    -   [ ] read non UTF-8/ ASCII filename: `actix_multipart::server::Field.content_disposition().get_filename_ext()`

-   [ ] ==dbpool== crate
    -   [ ] implement down/upload for json files
