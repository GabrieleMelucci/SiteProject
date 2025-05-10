from flask import Blueprint, render_template, redirect, url_for, request, flash
from flask_login import LoginManager, UserMixin, login_user, login_required, logout_user, current_user
from werkzeug.security import generate_password_hash, check_password_hash
from src.model import db, User

# Create the blueprint for the login management
login_bp = Blueprint('login_bp', __name__)

# LoginManager for the user session
login_manager = LoginManager()

@login_manager.user_loader
def load_user(user_id):
    return User.query.get(int(user_id))

# User Registration
@login_bp.route('/register', methods=['GET', 'POST'])
def register():
    if request.method == 'POST':
        username = request.form['username']
        password = request.form['password']
        hashed_password = generate_password_hash(password, method='sha256')

        # Create a new user
        new_user = User(username=username, password=hashed_password)
        db.session.add(new_user)
        db.session.commit()
        
        flash("Sign up successful.", "success")
        return redirect(url_for('login_bp.login'))

    return render_template('register.html')

# Login of the user
@login_bp.route('/login', methods=['GET', 'POST'])
def login():
    if request.method == 'POST':
        username = request.form['username']
        password = request.form['password']
        
        user = User.query.filter_by(username=username).first()
        
        if user and check_password_hash(user.password, password):
            login_user(user)
            flash("Login successful.", "success")
            return redirect(url_for('home'))  # Redirect to the homepage after
        else:
            flash("Wrong username or password, reinsert your credentials.", "danger")

    return render_template('login.html')

# Logout of the user
@login_bp.route('/logout')
@login_required
def logout():
    logout_user()
    flash("You have been disconnected.", "success")
    return redirect(url_for('login_bp.login'))
