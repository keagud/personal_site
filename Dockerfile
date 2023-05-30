FROM  python:3.11-bullseye
WORKDIR /app
COPY . .
RUN pip install -r requirements.txt && pip install gunicorn
CMD ["gunicorn"  , "-b", "0.0.0.0:8000", "app:app"]
