use oteryn_internal_api::playerdata::{Error, sessions};

fn zero_duration_segment_with_kill(end_reason: &str, final_segment: bool) -> String {
    format!(
        concat!(
            r#"{{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"game.test","#,
            r#""stream_generation":"019a5000-0000-7000-8000-000000000001","account_id":"019a5000-0000-7000-8000-000000000002","#,
            r#""source_revision":"1","record_kind":"HUNT_SEGMENT","record":{{"#,
            r#""session_id":"019a5000-0000-7000-8000-000000000003","segment_id":"019a5000-0000-7000-8000-000000000004","#,
            r#""segment_index":0,"character_id":"019a5000-0000-7000-8000-000000000005","#,
            r#""world_id":"019a5000-0000-7000-8000-000000000006","channel_id":"019a5000-0000-7000-8000-000000000007","#,
            r#""character_revision":"1","catalog_revision":"fixture.v1","started_at":"1793836800","ended_at":"1793836800","#,
            r#""duration_seconds":"0","end_reason":"{}","final_segment":{},"#,
            r#""experience_gained":"0","experience_lost":"0","level":20,"vocation_key":"vocation:knight","#,
            r#""places":[],"loot":[],"supplies":[],"damage_dealt":[],"damage_taken":[],"healing_done":"0","healing_received":"0","#,
            r#""monsters_killed":[{{"monster_key":"monster:rat","count":"1"}}],"rare_drop_count":0,"party":null}}}}"#,
        ),
        end_reason, final_segment,
    )
}

#[test]
fn accepts_zero_duration_area_exit_with_kill() {
    let raw = zero_duration_segment_with_kill("AREA_EXIT", false);
    let result = sessions::record(raw.as_bytes());
    assert!(
        result.is_ok(),
        "same-second AREA_EXIT may contain a kill: {result:?}"
    );
}

#[test]
fn rejects_zero_duration_session_end_with_kill() {
    let raw = zero_duration_segment_with_kill("SESSION_END", true);
    assert_eq!(sessions::record(raw.as_bytes()).err(), Some(Error::Invalid));
}
