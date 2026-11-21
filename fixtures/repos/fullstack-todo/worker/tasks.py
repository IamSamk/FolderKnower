import smtplib

from celery import Celery

celery = Celery("todo", broker="redis://localhost:6379/0")


@celery.task
def send_digest(user_id: int):
    with smtplib.SMTP("localhost") as smtp:
        smtp.sendmail("noreply@example.com", [f"user{user_id}@example.com"], "Your todos")
