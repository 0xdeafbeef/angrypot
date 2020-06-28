#!/usr/bin/env bash
sqlite3  passwords.db "create table if not exists passwords(password TEXT, count integer);create table if not exists logins(login TEXT, count integer);"