-- Add migration script here
ALTER TABLE purchaseable_roles DROP column only_one;
ALTER TABLE purchaseable_roles ADD column only_one boolean not null default false;
