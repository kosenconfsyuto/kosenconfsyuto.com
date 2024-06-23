/* document.addEventListener('DOMContentLoaded', function () {
    const slides = document.querySelector('.slides');
    const slideCount = document.querySelectorAll('.slide').length;
    const visibleSlides = 3;
    const slideWidth = 100 / visibleSlides;
    let currentIndex = 0;
    let isPlaying = true;
    let slideInterval;

    const prevButton = document.querySelector('.prev');
    const nextButton = document.querySelector('.next');
    const playPauseButton = document.querySelector('.play-pause');
    const navigation = document.querySelector('.navigation');

    function createNavigation() {
        for (let i = 0; i < slideCount; i++) {
            const dot = document.createElement('span');
            dot.classList.add('dot');
            if (i === currentIndex) {
                dot.classList.add('active');
            }
            dot.addEventListener('click', function () {
                goToSlide(i);
            });
            navigation.appendChild(dot);
        }
    }

    function updateNavigation() {
        const dots = document.querySelectorAll('.dot');
        dots.forEach((dot, index) => {
            if (index === currentIndex) {
                dot.classList.add('active');
            } else {
                dot.classList.remove('active');
            }
        });
    }

    function goToSlide(index) {
        // 調整：スライドが中央に来るように計算
        const offset = index - Math.floor(visibleSlides / 2);
        const startIndex = Math.max(0, Math.min(slideCount - visibleSlides, offset));
    
        // 一旦全てのスライドからクラスを削除
        const allSlides = document.querySelectorAll('.slide');
        allSlides.forEach(slide => {
            slide.classList.remove('slide-prev', 'slide-center', 'slide-next');
        });
    
        // インデックスに対応するスライドにクラスを付ける
        for (let i = 0; i < visibleSlides; i++) {
            const slideIndex = startIndex + i;
            if (slideIndex === index) {
                allSlides[slideIndex].classList.add('slide-center');
            } else if (slideIndex < index) {
                allSlides[slideIndex].classList.add('slide-prev');
            } else if (slideIndex > index) {
                allSlides[slideIndex].classList.add('slide-next');
            }
        }
    
        currentIndex = index;
        updateNavigation();
    }       

    function nextSlide() {
        if (currentIndex < slideCount - visibleSlides) {
            goToSlide(currentIndex + 1);
        } else {
            goToSlide(0);
        }
    }

    function prevSlide() {
        if (currentIndex > 0) {
            goToSlide(currentIndex - 1);
        } else {
            goToSlide(slideCount - visibleSlides);
        }
    }

    function playSlides() {
        slideInterval = setInterval(nextSlide, 2000);
        playPauseButton.innerHTML = '<img src="/icons/pause_24dp_FILL1_wght200_GRAD0_opsz24.svg" />';
    }

    function pauseSlides() {
        clearInterval(slideInterval);
        playPauseButton.innerHTML = '<img src="play_arrow_24dp_FILL0_wght200_GRAD0_opsz24.svg" />';
    }

    function togglePlayPause() {
        if (isPlaying) {
            pauseSlides();
        } else {
            playSlides();
        }
        isPlaying = !isPlaying;
    }

    prevButton.addEventListener('click', prevSlide);
    nextButton.addEventListener('click', nextSlide);
    playPauseButton.addEventListener('click', togglePlayPause);

    createNavigation();  // Create navigation dots
    goToSlide(0);  // 最初に1枚目のスライドが中央に来るようにする
    playSlides();  // Start the slide show
}); */
