from flask import Flask, render_template
from src.search import search_bp 

app = Flask(__name__)

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