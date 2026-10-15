# Oracle ARM64 qualification

Work in progress, governed by Platform issue #1639. These files prepare native
ARM64 images; they do not switch traffic or start a second authoritative Game
writer against NAS data.

The Game image uses an immutable Game source archive as its build context and
runs as numeric UID/GID 1001. Operations requiring root use separate one-off
containers. Platform and gateway use the existing reviewed Synology Dockerfiles
with a native ARM64 build target.

Qualified source inputs:

- Platform: `1194d5548760d9d773a1cebb182bc5986a6a42bf`
- Game: `87f770c87ce17518ba69f5effdbd04725d8a7d06`

Deployment remains incomplete until dedicated PostgreSQL TLS identities, native
admission, Character fencing, independent projection witnesses, backups and the
real login/create/list path have been verified on the isolated Oracle stack.
Do not attach live NAS databases or change public DNS before that qualification.
