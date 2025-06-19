-- Create sponsorTimes table
CREATE TABLE IF NOT EXISTS "sponsorTimes" (
    "videoID" TEXT NOT NULL,
    "startTime" REAL NOT NULL,
    "endTime" REAL NOT NULL,
    "votes" INTEGER NOT NULL DEFAULT 0,
    "locked" INTEGER NOT NULL DEFAULT 0,
    "incorrectVotes" INTEGER NOT NULL DEFAULT 0,
    "UUID" TEXT PRIMARY KEY,
    "userID" TEXT NOT NULL,
    "timeSubmitted" BIGINT NOT NULL,
    "views" INTEGER NOT NULL DEFAULT 0,
    "category" TEXT NOT NULL,
    "actionType" TEXT NOT NULL,
    "service" TEXT NOT NULL DEFAULT 'YouTube',
    "videoDuration" REAL NOT NULL DEFAULT 0,
    "hidden" INTEGER NOT NULL DEFAULT 0,
    "reputation" REAL NOT NULL DEFAULT 0,
    "shadowHidden" INTEGER NOT NULL DEFAULT 0,
    "hashedVideoID" TEXT NOT NULL,
    "userAgent" TEXT NOT NULL DEFAULT '',
    "description" TEXT NOT NULL DEFAULT ''
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS "idx_sponsorTimes_hashedVideoID" ON "sponsorTimes" ("hashedVideoID");
CREATE INDEX IF NOT EXISTS "idx_sponsorTimes_videoID" ON "sponsorTimes" ("videoID");
CREATE INDEX IF NOT EXISTS "idx_sponsorTimes_category" ON "sponsorTimes" ("category");
CREATE INDEX IF NOT EXISTS "idx_sponsorTimes_hidden_shadowHidden" ON "sponsorTimes" ("hidden", "shadowHidden");
CREATE INDEX IF NOT EXISTS "idx_sponsorTimes_votes" ON "sponsorTimes" ("votes");