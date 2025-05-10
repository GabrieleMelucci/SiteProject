from flask import Flask, render_template
from src.search import search_bp
from src.login import login_bp, login_manager  # Import login_manager from login.py
from src.model import db

app = Flask(__name__)

# Configure the SQLite database
app.config['SQLALCHEMY_DATABASE_URI'] = 'sqlite:///site.db'  # Imposta il percorso del DB SQLite
app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False  # Disabilita il tracking delle modifiche (ottimizzazione)
app.config['SECRET_KEY'] = 'supersecretkey'  # Cambia questa chiave con una chiave segreta sicura



# Initialize the database and the LoginManager
db.init_app(app)
login_manager.init_app(app)  # Initialize login_manager here

# Register the login blueprint
app.register_blueprint(login_bp, url_prefix='/auth')

# Register other blueprints (if any)
app.register_blueprint(search_bp, url_prefix='/api')

@app.route('/')
def home():
    return render_template('sitechinese.html')

@app.route('/about')
def about():
    return render_template('about.html')

@app.route('/changelog')
def changelog():
    return render_template('changelog.html')

@app.route('/login')
def login():
    return render_template('login.html')

@app.route('/privacy-policy')
def privacy():
    return render_template('privacy-policy.html')

@app.route('/search')
def search():
    return render_template('search.html')

@app.route('/terms-of-use')
def terms():
    return render_template('terms-of-use.html')

if __name__ == '__main__':
    app.run(debug=True, port=5000)