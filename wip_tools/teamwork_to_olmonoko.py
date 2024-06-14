# session_id = 'your-olmonoko-session-id'
api_base = 'https://olmonoko.ruta.fi/api'
# input_file = "your-teamwork-export.csv"

import csv
import requests

class TimeLog:
    def __init__(self, row):
        self.id = row[0]
        self.date = row[1]
        # date_time and end_date are in the format "29/04/2024 16:11"
        self.date_time = row[2]
        self.end_date_time = row[3]
        self.project = row[4]
        self.who = row[5]
        self.description = row[6]
        self.project_category = row[7]
        self.company = row[8]
        self.task_list = row[9]
        self.task = row[10]
        self.parent_task = row[11]
        self.is_sub_task = row[12]
        self.is_billable = row[13]
        self.invoice_number = row[14]
        self.hours = row[15]
        self.minutes = row[16]
        self.hours_decimal = row[17]

def csv_date_to_rf_date(date):
    # date is in the format '29/04/2024'
    return f'{date[6:10]}-{date[3:5]}-{date[0:2]}'
    

class OlmonokoEntry:
    def __init__(self, time_log):
        self.id = time_log.id
        self.summary = f"{time_log.project} {time_log.id}"
        self.description = time_log.description
        # starts_at is in the format '2024-04-29T16:11'
        start_date = csv_date_to_rf_date(time_log.date_time.split(' ')[0])
        start_time = time_log.date_time.split(' ')[1]
        self.starts_at = f'{start_date}T{start_time}'
        self.starts_at_tz = 3

        # ends_at is in the format '2024-04-29T16:11'
        # end_date = csv_date_to_rf_date(time_log.end_date_time.split(' ')[0])
        # end_time = time_log.end_date_time.split(' ')[1]
        # self.ends_at = f'{end_date}T{end_time}'

        # duration is in seconds
        self.duration = int(time_log.hours) * 3600 + int(time_log.minutes) * 60
        self.location = time_log.company
        self.priority = 9
        self.tags = f"work,teamwork,tw::project::{time_log.project}"

def read_csv(file):
    with open(file, newline='') as csvfile:
        reader = csv.reader(csvfile, delimiter=',')
        next(reader)
        for row in reader:
            # print("row: ", row)
            yield TimeLog(row)

def upload_to_olmonoko(entries):
    for olmonoko_entry in entries:
        print(f"Uploading entry {olmonoko_entry.id}")
        rq_path = f'{api_base}/event/local'
        cookies = {'session_id': session_id}
        form = {
            'summary': olmonoko_entry.summary,
            'description': olmonoko_entry.description,
            'starts_at': olmonoko_entry.starts_at,
            # 'ends_at': olmonoko_entry.ends_at,
            'starts_at_tz': olmonoko_entry.starts_at_tz,
            'duration': olmonoko_entry.duration,
            'location': olmonoko_entry.location,
            'priority': olmonoko_entry.priority,
            'tags': olmonoko_entry.tags
        }
        response = requests.post(rq_path, cookies=cookies, data=form)
        if not response.ok: 
            print(f"\tError: {response.status_code}")
        # print(response.text)

def convert(logs):
    return [OlmonokoEntry(log) for log in logs]

if __name__ == '__main__':
    logs = read_csv(input_file)
    entries = convert(logs)
    # for entry in entries:
    #     print(entry.__dict__)
    upload_to_olmonoko(entries)

