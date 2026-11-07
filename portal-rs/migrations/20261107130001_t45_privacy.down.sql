ALTER TABLE character_profiles ALTER COLUMN show_guild SET DEFAULT true;
ALTER TABLE character_profiles DROP COLUMN comment, DROP COLUMN hidden, DROP COLUMN show_account, DROP COLUMN show_status, DROP COLUMN show_house, DROP COLUMN show_skills, DROP COLUMN show_deaths, DROP COLUMN show_kills, DROP COLUMN show_online_time;
ALTER TABLE accounts DROP COLUMN character_profiles_hidden;
