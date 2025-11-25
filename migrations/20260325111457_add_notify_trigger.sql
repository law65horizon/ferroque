-- Add migration script here
CREATE OR REPLACE FUNCTION notify_job_inserted()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('ferroque_jobs', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER jobs_inserted_trigger
AFTER INSERT ON jobs
FOR EACH ROW
EXECUTE FUNCTION notify_job_inserted();