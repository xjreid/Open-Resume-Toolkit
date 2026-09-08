"""Independent formatting oracle for synthetic output audits, never an importer."""

MONTHS = ("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")


def date_text(record):
    def calendar(value):
        if value is None:
            return ""
        month = value["month"]
        text = f"{MONTHS[month - 1]} {value['year']}" if month is not None else str(value["year"])
        return f"Expected {text}" if value["expected"] else text

    start = calendar(record["start"])
    end_value = record["end"]
    end = "Present" if end_value and end_value["kind"] == "present" else calendar(end_value["value"] if end_value else None)
    value = "–".join(part for part in (start, end) if part)
    label = record["label"].strip()
    return f"{label}: {value}" if label and value else value
