#! /bin/bash

use admin db.createUser( { user: "rootuser", pwd: "rootpass", roles: [ { role: "userAdminAnyDatabase", db: "admin" } ] } )