function changetheme(themename) {
    var html = document.documentElement;

    if (themename === 'dark') {
        html.classList.add('dark-theme');
        html.classList.remove('light-theme', 'system-theme');
        localStorage.setItem('theme', 'dark');
    } else if (themename === 'light') {
        html.classList.add('light-theme');
        html.classList.remove('dark-theme', 'system-theme');
        localStorage.setItem('theme', 'light');
    } else if (themename === 'system') {
        html.classList.add('system-theme');
        html.classList.remove('dark-theme', 'light-theme');
        localStorage.setItem('theme', 'system');
    }
}

document.addEventListener('DOMContentLoaded', function() {
    var theme = localStorage.getItem('theme');
    if (theme !== null) {
        changetheme(theme);
    } else {
        localStorage.setItem('theme', 'system');
    }
});