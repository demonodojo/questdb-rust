-- Written by QuestDB 10.1.0 (e11c837f8a169b533540bd6fa57ef3d520b68ad9)
-- with cairo.wal.enabled.default=false. The second day directory is 2020-01-03.1.
CREATE TABLE days (ts TIMESTAMP, id LONG, note STRING, location SYMBOL) TIMESTAMP(ts) PARTITION BY DAY;
INSERT INTO days VALUES ('2020-01-02T03:04:05.000000Z', 1, 'alpha', 'madrid');
INSERT INTO days VALUES ('2020-01-02T23:00:00.000000Z', 2, NULL, NULL);
INSERT INTO days VALUES ('2020-01-03T00:00:00.000000Z', 3, 'beta', 'madrid');
INSERT INTO days VALUES ('2020-01-03T12:30:00.000000Z', NULL, '東京', '東京');
