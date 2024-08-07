# OLMONOKO

> [!NOTE]
> Still very much work-in-progress, see [TODO.md](olmonoko-backend/TODO.md) for a list of planned features.
> 
> The current frontend is intended to be a placeholder and there is no complete api yet.

```mermaid
graph TD
    subgraph RS[Remote ics sources]
        RS1[example.com/calendar.ics]
        RS2[another.com/events.ics]
        subgraph P[''Plugins'']
            RS3[Bank transactions]
            RS4[Photo Gallery]
            RS5[Timesheets]
            RS6[Location History]
        end
    end
    subgraph EC[Calendar software]
            GCal[Google Calendar]
            Outlook
            ACal[Apple Calendar]
    end
    open[Other external services]

    sync(fetched every x minutes)
    RS1-->sync
    RS2-->sync
    RS3-->sync
    RS4-->sync
    RS5-->sync
    RS6-->sync

    subgraph O[OLMONOKO]
        api
        subgraph OE[Events]
            RE[Remote events]
            LE[Local Events]
            subgraph AT[Attachments]
                Bills
            end
        end
        OV[Export links]
        OE-->OV
    end
    O-->PN[Push/Email Notifications]

    sync--import templates-->RE

    api(api)
    manual[users]-->api
    integrations[integrations]-->api
    api-->LE
    api-->AT

OV--.ics-->EC
OV--api-->open
```
