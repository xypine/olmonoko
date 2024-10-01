- [x] fix RRULEs being escaped
- [x] automatically sync calendars every X hours / days / ?
- [x] manual events / inbuilt calendar source
- [x] api for local events
- [x] ui for creating local events
- [x] ui for viewing local events
- [x] ui for removing / updating local events
- [x] bills
- [-] barcode scanner for bills
  - [ ] barcode scanning from images sucks, find a better way?
- [x] add all_day to events
- [x] fix newlines in produced ics
- [x] add date controls to default view
- [x] inject attachment descriptions at runtime
- [x] chrono cleanup / remove usage of any deprecated functions
- [-] maybe remove payee fields from bills (bloat)
- [ ] add dedicated ui for attachments (bills etc)
- [x] priority for events, default from source
- [x] allow users to filter which events they want to see in their calendar / integrate with priority system
- [x] privacy / sensitive information system
  - [ ] features from the priority system
    - Store as integer, allowing multiple levels
    - Maybe the following:
      1. [Encrypted/SA] Top secret
      2. [Encrypted/SA] Private
      3. Not included in calendar exports by default
      4. no functional difference to 6 in normal ui, "busy"-level data in api and export links if not otherwise specified in export link / api key
      5. default - private
      6. no functional difference to 5
      7. visible publicly as "busy"-block
      8. visible publicly as "busy"-block, name shown to connections
      9. visible publicly
  - [ ] encryption / special access for private events
    - client side or serverside with client supplying the key through POST request? Maybe allow for both and express this in UI and docs?
    - not accessible through session or api key alone, needs encryption key
    - would maybe need some partitioning so that all of the user's private events would not need to be decrypted and kept in memory for access to only this weeks private events?
- [x] allow multiple export links
- [x] add min_priority to export links
- [-] keep track of uids so that we can present lately added / modified events
- [-] import rules
- [-] change import templates to be per-user, don't modify original data
  - either at runtime (prob not worth it) or import
- [ ] allow "pinning" of events / moving them to the local calendar
  - [ ] show modifications since pinning
- [-] more homepage views / week overview
- [x] week overview: handle overlapping events
- [-] allow users to set a default timezone for ui
- [-] local event tags
- [x] local event filters
  - [x] ui
- [-] local event bulk delete
- [ ] soft delete for local events
- [-] local event RRULEs
  - [ ] Convert local events to an event source generated locally
- [-] vim keybindings
- [-] "scrubbable navigation with heatmap"
- [ ] import ics files that aren't hosted anywhere
- [ ] add proper error handling to frontend
- [ ] i18n
- [ ] track user attendance of events / maybe automatically based on location?
  - [ ] addressbook
  - [-] native app
    - [ ] geofencing
    - [ ] clock-in-n-out
    - [ ] notifications
- [ ] api keys
- [ ] headless mode / system user dictated by a config file
- [ ] turn persist_events automatically off / on based on source behaviour
- [-] try automatically caching internal links on current page
- [ ] automatic geolocation + manual override
- [ ] remind when to leave / maps integration
- [ ] clock-in-n-out
  - [-] teamwork integration
    - [x] import
    - [ ] export
- [ ] more default themes, custom theme support
  - maybe try #D0A657
- [ ] automatic weather location based on planned event attendance
  - [ ] allow for comparing location checks in import templates
- [ ] automatically add events for all pictures in gallery / photos / photoprism integration
- [ ] automatic syncing between instances / per user
- [ ] embeddable export links / public calendar views with privacy options
- [ ] countdown pages / something more generic?
- [ ] other parts of the ICalendar spec
  - [ ] TODOs
