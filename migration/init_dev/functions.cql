CREATE OR REPLACE FUNCTION bucket_now()
    CALLED ON NULL INPUT
    RETURNS timestamp
    LANGUAGE LUA
    AS '
local current_time = os.time()
local current_date = os.date("*t", current_time)
local current_weekday = current_date.wday
local days_since_last_monday = current_weekday - 2

if days_since_last_monday < 0 then
    days_since_last_monday = days_since_last_monday + 7
end

local last_monday_date = {
    year = current_date.year,
    month = current_date.month,
    day = current_date.day - days_since_last_monday,
    hour = 0,
    min = 0,
    sec = 0,
}

return os.time(last_monday_date)
';