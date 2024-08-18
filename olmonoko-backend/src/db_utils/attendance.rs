use olmonoko_common::models::attendance::{
    Attendance, AttendanceEvent, NewAttendance, RawAttendance,
};
use sqlx::{Executor, Postgres};

#[allow(async_fn_in_trait)]
pub trait DBWrite {
    async fn write<C>(&self, conn: &mut C) -> Result<Option<Attendance>, sqlx::Error>
    where
        for<'e> &'e mut C: Executor<'e, Database = Postgres>;
}

impl DBWrite for NewAttendance {
    async fn write<C>(&self, conn: &mut C) -> Result<Option<Attendance>, sqlx::Error>
    where
        for<'e> &'e mut C: Executor<'e, Database = Postgres>,
    {
        let (local_event_id, remote_event_id) = match self.event_id {
            AttendanceEvent::Local(id) => (Some(id), None),
            AttendanceEvent::Remote(id) => (None, Some(id)),
        };
        if self.planned || self.actual {
            let raw = sqlx::query_as!(
                RawAttendance,
                r#"
                INSERT INTO attendance
                    ( user_id, local_event_id, remote_event_id, planned, actual )
                VALUES
                    ( $1, $2, $3, $4, $5 )
                ON CONFLICT(user_id, coalesce(local_event_id, -1), coalesce(remote_event_id, -1)) DO UPDATE SET
                    planned = excluded.planned,
                    actual = excluded.actual,
                    updated_at = EXTRACT(EPOCH FROM NOW())*1000
                RETURNING *
            "#,
                self.user_id,
                local_event_id,
                remote_event_id,
                self.planned,
                self.actual,
            )
            .fetch_one(&mut *conn)
            .await?;

            return Ok(Some(Attendance::from(raw)));
        } else {
            sqlx::query!("DELETE FROM attendance WHERE user_id = $1 AND (local_event_id = $2 OR $2 IS NULL) AND (remote_event_id = $3 OR $3 IS NULL)", self.user_id, local_event_id, remote_event_id)
            .execute(&mut *conn)
            .await?;
        }

        Ok(None)
    }
}
