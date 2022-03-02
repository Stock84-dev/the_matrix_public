CREATE OR REPLACE FUNCTION file.delete_research_results()
    RETURNS TRIGGER
    LANGUAGE 'plpgsql'
AS
$$
BEGIN
    EXECUTE FORMAT('DROP TABLE research_result_blocks_%s;', OLD.id);
END
$$;

DROP TRIGGER IF EXISTS delete_research_results on files;
CREATE TRIGGER delete_research_results
    BEFORE DELETE
    ON files
    FOR EACH ROW
EXECUTE PROCEDURE file.delete_research_results();
