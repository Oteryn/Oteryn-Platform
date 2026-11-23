use super::*;
use oteryn_identity::staff_account::{self, Field};

pub(super) const fn permission(field: Field) -> Permission {
    match field {
        Field::Email => Permission::EditPlayerEmail,
        Field::DisplayName => Permission::EditPlayerDisplayName,
        Field::Locale => Permission::EditPlayerLocale,
        Field::PublicAccountAssociation => Permission::EditPlayerPublicAccountAssociation,
        Field::PublicStatusVisible => Permission::EditPlayerPublicStatusVisible,
        Field::CharacterProfilesHidden => Permission::EditPlayerCharacterProfilesHidden,
    }
}

impl From<staff_account::Error> for Error {
    fn from(error: staff_account::Error) -> Self {
        match error {
            staff_account::Error::BadInput => Self::BadInput,
            staff_account::Error::Conflict => Self::Conflict,
            staff_account::Error::Storage(error) => Self::Storage(error),
        }
    }
}

async fn player_only(conn: &mut PgConnection, actor: Uuid, id: Uuid) -> Result<bool, Error> {
    if actor == id {
        return Ok(false);
    }
    Ok(
        sqlx::query_scalar("SELECT NOT EXISTS(SELECT FROM account_roles WHERE account_id=$1)")
            .bind(id)
            .fetch_one(conn)
            .await?,
    )
}

impl<A: AccountStateReader, I: Accounts> Players<A, I> {
    pub(super) async fn editable_fields(
        &self,
        context: CheckContext,
        id: Uuid,
    ) -> Result<Vec<(Field, String)>, Error> {
        let mut fields = Vec::new();
        for field in Field::ALL {
            match self.auth.check(context, permission(field)).await {
                Ok(()) => fields.push(field),
                Err(crate::rbac::Error::Denied) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        if fields.is_empty() {
            return Ok(Vec::new());
        }
        let mut conn = self.pool.acquire().await?;
        if !player_only(&mut conn, context.account, id).await? {
            return Ok(Vec::new());
        }
        let Some(details) = staff_account::details(&mut conn, id).await? else {
            return Ok(Vec::new());
        };
        Ok(fields
            .into_iter()
            .map(|field| (field, details.value(field).to_owned()))
            .collect())
    }

    pub(super) async fn edit(
        &self,
        context: CheckContext,
        id: Uuid,
        field: Field,
        value: &str,
        reason: &str,
    ) -> Result<bool, Error> {
        let field_permission = permission(field);
        self.check(context, Permission::ManageSupportTickets)
            .await?;
        self.check(context, field_permission).await?;
        valid_id(id)?;
        valid_text(reason, 500)?;
        let edit = staff_account::Edit::parse(field, value)?;
        self.target(id).await?;
        if matches!(field, Field::Email | Field::Locale) {
            self.admit(context.account, "staff.players.contact", 6)
                .await?;
        }
        let mut tx = self.pool.begin().await?;
        crate::rbac::mutation_budget(&mut tx).await?;
        edit.lock_address(&mut tx).await?;
        crate::rbac::authorize_mutation(
            &mut tx,
            &[id],
            &self.auth,
            context,
            &[
                Permission::Access,
                Permission::ManageSupportTickets,
                field_permission,
            ],
        )
        .await?;
        if !player_only(&mut tx, context.account, id).await? {
            return Err(Error::Denied);
        }
        let current = staff_account::details(&mut tx, id)
            .await?
            .ok_or(Error::Conflict)?;
        let changed = edit.apply(&mut tx, id, &current).await?;
        if changed {
            append(
                &mut tx,
                context,
                "players.account.edited",
                id,
                json!({"field":field.key(), "reason":reason.trim()}),
            )
            .await?;
        }
        self.check_in(&mut tx, context, Permission::ManageSupportTickets)
            .await?;
        self.check_in(&mut tx, context, field_permission).await?;
        tx.commit().await?;
        Ok(changed)
    }
}
